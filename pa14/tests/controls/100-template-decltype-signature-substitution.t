// N3485 7.1.6.2p4, 14.5.6.1p5 and 14.6p10: dependent result operands
// substitute once with definition-owned parameter names and lookup; expect exit 0
long pick(double) { return 7; }
template<class T> auto result(T value) -> decltype(pick(value)) { return pick(value); }
int pick(int) { return 3; }
template<class U> auto result(U renamed) -> decltype(pick(renamed));
template<class T> auto identity(T value) -> decltype(value);
template<class U> auto identity(U renamed) -> decltype(renamed) { return renamed; }
template<class T, class U> auto difference(T a, U b) -> decltype(b - a) { return b - a; }
template<class T, class U = decltype(T())> U default_type(T value) { return value; }
int main()
{
  return result(0) == 7 && identity(9) == 9 && difference(2, 9L) == 7
    && default_type(3) == 3 && default_type(4L) == 4 ? 0 : 1;
}
