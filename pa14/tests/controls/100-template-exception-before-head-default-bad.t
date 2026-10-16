// N3485 15.4p3 and 14.1p12: exception agreement precedes merging a
// redeclaration's repeated default template argument; expect reject.
template<class T = int> int choose(T) noexcept;
template<class U = int> int choose(U);
int main() { return 0; }
