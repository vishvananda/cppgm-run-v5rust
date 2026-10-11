struct A { int value; };
struct B : virtual A {};
struct D : B {};

template<class T, class U>
int add(T & left, U & right)
{
  return left.value + right.value;
}

int main()
{
  D left;
  D right;
  left.value = 2;
  right.value = 5;
  return add<B, B>(left, right) - 7;
}
