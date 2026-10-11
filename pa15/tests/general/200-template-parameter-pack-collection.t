// HHC-045
template<class... Args>
struct Box {};

Box<int, long> b;

int main() { return 0; }
