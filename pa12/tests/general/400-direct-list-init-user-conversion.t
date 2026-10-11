// One conversion of a constructor argument is valid outside class copy initialization.
struct A { int value; A(int n) : value(n) {} };
struct X { operator int() const { return 7; } };
struct Y { operator A() const { return A(9); } };
int main()
{
  X source;
  A direct(source);
  A direct_list{source};
  A copy_list = {source};
  Y producer;
  A converted = producer;
  A class_direct_list{producer};
  A class_copy_list = {producer};
  return direct.value == 7 && direct_list.value == 7 &&
         copy_list.value == 7 && converted.value == 9 &&
         class_direct_list.value == 9 && class_copy_list.value == 9 ? 0 : 1;
}
