struct Packet { int head; int values[]; };
int main() { Packet value = {1, {2}}; return value.head; }
