struct C
{
  int & f(const int &);
  int field;
};

using member_function_ptr = int & (C::*)(const int &);
using member_object_ptr = int C::*;

int main()
{
  member_function_ptr fn = nullptr;
  member_object_ptr field = nullptr;
  return fn == nullptr && field == nullptr ? 0 : 1;
}
