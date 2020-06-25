
use regex::Regex;
use std::{io, mem, ptr};
use std::ffi::{CStr, CString, OsStr};
use std::str;
use winapi::shared::guiddef::*;
use winapi::shared::ntdef::CHAR;
use winapi::shared::winerror::*;
use winapi::shared::minwindef::*;
use winapi::shared::hidclass::*;
use winapi::um::setupapi::*;
use winapi::um::cguid::*;
use winapi::um::errhandlingapi::GetLastError;
use std::fmt::{Display, Formatter, Error};

pub struct USBInfo {
    /// The hardware device type that exposes this port
    pub path: String,
    pub product_id: u16,
    pub vendor_id: u16,
    pub product_string: Option<String>,
    pub serial_number_string: Option<String>,
    pub dev_inst: Option<BOOL>,
    pub pdo_name: Option<String>
}


struct HidDevices {
    /// Handle to a device information set.
    hdi: HDEVINFO,

    /// Index used by iterator.
    dev_idx: DWORD,
}

impl HidDevices {
    pub fn new(guid: &GUID) -> Self {
        HidDevices {
            hdi: unsafe { SetupDiGetClassDevsA(guid, ptr::null(), ptr::null_mut(), DIGCF_DEVICEINTERFACE) },
            dev_idx: 0,
        }
    }
}

impl Iterator for HidDevices {
    type Item = HidDevice;

    fn next(&mut self) -> Option<HidDevice> {
        let mut hid_dev = HidDevice {
            hdi: self.hdi,
            devinfo_data: SP_DEVINFO_DATA {
                cbSize: mem::size_of::<SP_DEVINFO_DATA>() as DWORD,
                ClassGuid: GUID_NULL,
                DevInst: 0,
                Reserved: 0,
            },
            devinfo_interface_data: SP_DEVICE_INTERFACE_DATA  {
                cbSize: mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as DWORD,
                InterfaceClassGuid: GUID_NULL,
                Flags: 0,
                Reserved: 0,
            },
            devinfo_interface_detail: SP_DEVICE_INTERFACE_DETAIL_DATA_A {
                cbSize: mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_A>() as DWORD,
                DevicePath: [64],
            },
        };
    
        let res =
            unsafe { SetupDiEnumDeviceInfo(self.hdi, self.dev_idx, &mut hid_dev.devinfo_data) };
        let interface_res =
            unsafe { SetupDiEnumDeviceInterfaces(self.hdi, &mut hid_dev.devinfo_data, &GUID_DEVINTERFACE_HID, self.dev_idx, &mut hid_dev.devinfo_interface_data) };

        if interface_res == TRUE {
            
            let mut dwRequiredSize:DWORD = 0;
            let ret = unsafe { SetupDiGetDeviceInterfaceDetailA(self.hdi, 
                &mut hid_dev.devinfo_interface_data, 
                ptr::null_mut(), 
                0, 
                &mut dwRequiredSize, 
                ptr::null_mut()) };

             
            // let mut size:DWORD = mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_A>() as DWORD;
            let ret = unsafe { SetupDiGetDeviceInterfaceDetailA(self.hdi, 
                &mut hid_dev.devinfo_interface_data, 
                &mut hid_dev.devinfo_interface_detail, 
                dwRequiredSize, 
                ptr::null_mut(), 
                ptr::null_mut())};
        }
        
        if res == FALSE {
            None
        } else {
            self.dev_idx += 1;
            Some(hid_dev)
        }
    }
}

impl Drop for HidDevices {
    fn drop(&mut self) {
        // Release the PortDevices object allocated in the constructor.
        unsafe {
            SetupDiDestroyDeviceInfoList(self.hdi);
        }
    }
}

struct HidDevice {
    /// Handle to a device information set.
    hdi: HDEVINFO,

    /// Information associated with this device.
    pub devinfo_data: SP_DEVINFO_DATA,
    pub devinfo_interface_data: SP_DEVICE_INTERFACE_DATA,
    pub devinfo_interface_detail: SP_DEVICE_INTERFACE_DETAIL_DATA_A
}

impl HidDevice {
    fn instance_id(&mut self) -> Option<String> {
        let mut result_buf = [0i8; MAX_PATH];
        let res = unsafe {
            SetupDiGetDeviceInstanceIdA(
                self.hdi,
                &mut self.devinfo_data,
                result_buf.as_mut_ptr(),
                (result_buf.len() - 1) as DWORD,
                ptr::null_mut(),
            )
        };
        if res == FALSE {
            // Try to retrieve hardware id property.
            self.property(SPDRP_HARDWAREID)
        } else {
            let end_of_buffer = result_buf.len() - 1;
            result_buf[end_of_buffer] = 0;
            Some(unsafe {
                CStr::from_ptr(result_buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            })
        }
    }

    pub fn hardware_id(&mut self) -> String {
        if let Some(hardware_id) = self.instance_id() {
            return hardware_id;
        }
        return String::from("")
    }

    pub fn property(&mut self, property_id: DWORD) -> Option<String> {
        let mut result_buf: [CHAR; MAX_PATH] = [0; MAX_PATH];
        let res = unsafe {
            SetupDiGetDeviceRegistryPropertyA(
                self.hdi,
                &mut self.devinfo_data,
                property_id,
                ptr::null_mut(),
                result_buf.as_mut_ptr() as PBYTE,
                (result_buf.len() - 1) as DWORD,
                ptr::null_mut(),
            )
        };
        if res == FALSE {
            if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
                return None;
            }
        }
        let end_of_buffer = result_buf.len() - 1;
        result_buf[end_of_buffer] = 0;
        Some(unsafe {
            CStr::from_ptr(result_buf.as_ptr())
                .to_string_lossy()
                .into_owned()
        })
    }
}

pub fn get_usb_info() -> Vec::<USBInfo> {
    let mut usbs = Vec::<USBInfo>::new();

    let hid_devices = HidDevices::new(&GUID_DEVINTERFACE_HID);
    
    for mut hid_device in hid_devices {
    
        let re = Regex::new(concat!(
            r"VID_(?P<vid>[[:xdigit:]]{4})",
            r"[&+]PID_(?P<pid>[[:xdigit:]]{4})",
            r"([\\+](?P<serial>\w+))?"
        )).unwrap();
        if let Some(caps) = re.captures(&hid_device.hardware_id()) {
            if let Ok(vid) = u16::from_str_radix(&caps[1], 16) {
                if let Ok(pid) = u16::from_str_radix(&caps[2], 16) {
                    let serial_number = caps.get(4).map(|m| m.as_str().to_string()).unwrap();
                    let product_string = hid_device.property(SPDRP_FRIENDLYNAME);
                    //Sorry in Rust I don't know how to get i want 
                    // devinfo_data: SP_DEVINFO_DATA {
                    //     cbSize: mem::size_of::<SP_DEVINFO_DATA>() as DWORD,
                    //     ClassGuid: GUID_NULL,
                    //     DevInst: 0,
                    //     Reserved: 0,
                    // },
                    // devinfo_interface_data: SP_DEVICE_INTERFACE_DATA  {
                    //     cbSize: mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as DWORD,
                    //     InterfaceClassGuid: GUID_NULL,
                    //     Flags: 0,
                    //     Reserved: 0,
                    // },
                    // devinfo_interface_detail: SP_DEVICE_INTERFACE_DETAIL_DATA_A {
                    //     cbSize: mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_A>() as DWORD,
                    //     DevicePath: [64],
                    // },
                    // 
                    // in C++, We can use devinfo_data->DevInst to get dev inst,
                    // And Device Path also. But in rust i don't know how to do that. 
                    usbs.push(USBInfo {
                        // Sorry in Rust I don't know how to get like this 
                        path: String::from(""), 
                        product_id: pid,
                        vendor_id: vid,
                        product_string: product_string,
                        serial_number_string: Some(serial_number),
                        dev_inst: Some(FALSE),
                        pdo_name: Some(String::from("")), // I don't pad is what?
                    });
                }
            }
        }

    }
    return usbs
}

