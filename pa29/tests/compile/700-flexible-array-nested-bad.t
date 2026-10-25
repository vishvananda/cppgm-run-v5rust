struct Packet { int head; int values[]; };
struct Outer { Packet packet; };
Outer value = {{1, {2}}};
