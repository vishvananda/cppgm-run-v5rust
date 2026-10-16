// N3485 8.3.6, 14.6.4.2: qualified definitions and renamed parameters keep
// default lookup at the introducing declaration; expect exit 0
namespace N {
int value = 4;
long select(double) { return 7; }
template<class T> int use(T, int x = value + select(T()));
int select(int) { return 13; }
}
template<class U> int N::use(U, int x) { return x; }
template<class T> int nested(T x, int y = N::use(T())) { return x + y; }
int main() {
  int value = 99;
  return N::use(1) == 11 && nested(2) == 13 && value == 99 ? 0 : 1;
}
