mod dag;
mod utils;

use dag::BaseUnit;
use dag::Dag;
use uuid::Uuid;

fn main() {
    let json_str = r#"
    {
        "name":"example",
        "dag": [
            {
                "id":"5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
                "name":"ComputeUnit1",
                "node_type": "ComputeUnit",
                "dependency": [],
                "cmd": ["ls"],
                "image":""
            },
            {
                "id":"1193c01b-9847-4660-9ea1-34b66f7847f4",
                "name":"Channel2",
                "node_type": "Channel",
                "dependency": ["5c42b900-a87f-45e3-ba06-c40d94ad5ba2"],
                "image":""
            },
            {
                "id":"353fc5bf-697e-4221-8487-6ab91915e2a1",
                "name":"ComputeUnit3",
                "node_type": "ComputeUnit",
                "dependency": ["1193c01b-9847-4660-9ea1-34b66f7847f4"],
                "cmd": ["ls"],
                "image":""
            }
        ]
    }
"#;

    println!("{}", Uuid::new_v4());

    let result = Dag::from_json(json_str).unwrap();
    result.iter().for_each(|v| {
        println!("{}", v.name());
    });
}
