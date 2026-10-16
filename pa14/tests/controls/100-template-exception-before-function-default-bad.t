// N3485 15.4p3 and 8.3.6p4: exception agreement precedes the prohibition
// on a later function-template declaration adding a default; expect reject.
template<class T> int choose(T, int) noexcept;
template<class U> int choose(U, int = 1);
int main() { return 0; }
