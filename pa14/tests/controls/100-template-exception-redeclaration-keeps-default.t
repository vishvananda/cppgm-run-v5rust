// N3485 15.4p3 and 14.7.1p12: equivalent exception patterns agree while
// a function-template default remains an unused pattern; expect exit 0.
template<class T> int choose(T, int = sizeof(T)) noexcept(sizeof(T) == 4);
template<class U> int choose(U, int) noexcept(sizeof(U) == 4);
int main() { return 0; }
