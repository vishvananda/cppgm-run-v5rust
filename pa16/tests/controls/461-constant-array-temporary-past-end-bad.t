// N3485 5.19p2: a temporary's one-past element cannot be read.
// expect reject
typedef int Pair[2];
constexpr int value = Pair{3, 4}[2];
int main() { return value; }
