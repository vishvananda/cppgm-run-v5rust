// N3485 15.4p3, 14.1p12 and 8.3.6p4: different dependent exception
// patterns conflict before repeated head and function defaults; expect reject.
template<class T = int> int choose(T, int = 1) noexcept(sizeof(T) == 4);
template<class U = int> int choose(U, int = 2) noexcept(sizeof(U) == 8);
int main() { return 0; }
