struct Packet { int head; int values[]; };
int runtime() { return 3; }
Packet value = {1, {runtime()}};
