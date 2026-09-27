
const CRTP = struct {
    header: u8,
    payload: [30]u8
};

const Command = struct {
    roll: f32,
    pitch: f32,
    yaw: f32,
    thrust: u32

    fn asBytes(self: *Command) *[14]u8 {

    }
};



