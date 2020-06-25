mod hid;
use std::fmt::{Display, Formatter, Error};
use std::process;

pub struct NumVec(Vec<hid::USBInfo>);

impl std::fmt::Display for NumVec {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut comma_separated = String::new();
        comma_separated.push_str(
            "hid devices: ["
        );
        for num in &self.0[0..self.0.len() - 1] {
            comma_separated.push_str(&num.to_string());
            comma_separated.push_str(",");
        }

        comma_separated.push_str(&self.0[self.0.len() - 1].to_string());
        comma_separated.push_str(
            "\n]"
        );
        write!(f, "{}", comma_separated)
    }
}

impl std::fmt::Display for hid::USBInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "
        HD:
            path: {:?},
            product_id: {:?},
            vendor_id: {:?},
            product_string:{:?},
            serial_number_string:{:?},
            dev_inst:{:?},
            pdo_name:{:?}
        ", self.path, self.product_id, self.vendor_id, self.product_string, self.serial_number_string,
    self.dev_inst, self.pdo_name)
    }
}


fn run_app() -> Result<(), ()> {
    // Application logic here
    let usbs = hid::get_usb_info();
    println!("{}", NumVec(usbs));
    Ok(())
}

fn main() {
    std::process::exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}