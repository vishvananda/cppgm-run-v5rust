// expect exit 0
template<class T> int array(T x[3]) { return x[0]; }
template<class T> int function(T x(int)) { return x(2); }
int twice(int value) { return value * 2; }
int main() {
  int values[3] = {3};
  return array(values) + function(twice) - 7;
}
