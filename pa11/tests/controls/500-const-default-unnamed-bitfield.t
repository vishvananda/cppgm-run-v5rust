// expect exit 0
struct Holder { int : 0; };
const Holder value;
int main() { return 0; }
