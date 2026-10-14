// expect exit 0
// N3485 4.2/4.3: an operator's array/function reference result decays by address.
int identity(int n) { return n + 1; }
struct Functions {
  int (&operator*())(int) { return identity; }
};
struct Array {
  int values[3];
  int (&operator*())[3] { return values; }
};
int main() {
  Functions functions;
  Array array{{2, 3, 4}};
  int (*function)(int) = *functions;
  int *values = *array;
  return (*functions)(2) == 3 && function(3) == 4
      && (*array)[1] == 3 && values[2] == 4 ? 0 : 1;
}
