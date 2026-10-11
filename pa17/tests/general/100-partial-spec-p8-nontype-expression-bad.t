// N3485 14.5.5p8 (first bullet, example A<I+5, I*2>): a specialized non-type
// argument shall not be a non-trivial expression of a parameter; expect reject
template<int I, int J> struct A {};
template<int I> struct A<I+5, I*2> {};
int main() { return 0; }
