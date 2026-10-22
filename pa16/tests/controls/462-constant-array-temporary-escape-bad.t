// N3485 5.19p3, 12.2: the temporary has no static storage duration.
// expect reject
typedef int Pair[2];
constexpr const int* escaped = &Pair{3, 4}[0];
int main() { return *escaped; }
