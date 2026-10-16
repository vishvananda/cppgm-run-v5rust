// N3485 13.5.7p1, 14.8.2.1: postfix syntax deduces the operand's type;
// the final int is implicit and does not require a default argument.
// expect exit 0
struct Counter { int value; };
template<class T> int operator++(T& counter, int)
{
  return counter.value++;
}
template<class T> int operator--(T& counter, int)
{
  return counter.value--;
}
int main()
{
  Counter counter = { 2 };
  int first = counter++;
  int second = counter--;
  return first == 2 && second == 3 && counter.value == 2 ? 0 : 1;
}
