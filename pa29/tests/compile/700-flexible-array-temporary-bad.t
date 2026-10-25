struct Packet { int head; int values[]; };
int consume(const Packet&);
int main() { return consume(Packet{1, {2}}); }
