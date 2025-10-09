use std::io;

trait Interaction {
    fn useitem(&self);   
}

struct Cubeitem{
        
}

impl Interaction for Cubeitem {
    fn useitem(&self){
        println!("interaction use")
    }
}

const CUBE:Cubeitem = Cubeitem{};

pub fn main(){
    let mut input=String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_n) => {
            if input.trim()=="e" {
                CUBE.useitem();
                println!("test");
            }
        }
        Err(error) => {
            println!("you REALLY fuhhed something up, {}",error.to_string())
        }
    }
    if input == "e" {
        CUBE.useitem();
    }
}

