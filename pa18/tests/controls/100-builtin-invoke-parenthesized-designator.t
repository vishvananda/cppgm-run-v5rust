// expect exit 0
// PA18: parenthesized callable names reuse their existing designation facts.
int function(int value) { return value + 1; }
long function(long value) { return value + 2; }
template<class T> T identity(T value) { return value; }

namespace associated {
  struct argument {};
  int identity(argument) { return 9; }
}
template<class T> int select(T) { return 3; }

int main() {
  int (*pointer)(int) = function;
  associated::argument argument;
  return __builtin_invoke(((function)), 2) == 3
      && __builtin_invoke(((identity<int>)), 4) == 4
      && __builtin_invoke(((pointer)), 5) == 6
      && __builtin_invoke(((select<associated::argument>)), argument) == 3
      && __builtin_invoke(((function)), __builtin_invoke(((pointer)), 6)) == 8
      ? 0 : 1;
}
