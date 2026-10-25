struct Packet { int head; int values[]; };
struct Outer { Packet packet = {1, {2}}; };
