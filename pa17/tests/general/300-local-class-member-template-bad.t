// N3485 14.5.2p2 (and 14.5.4p7 for friend templates): a local class shall not
// have member templates; expect reject
int main() { struct L { template<class T> int f(T) { return 0; } }; return 0; }
