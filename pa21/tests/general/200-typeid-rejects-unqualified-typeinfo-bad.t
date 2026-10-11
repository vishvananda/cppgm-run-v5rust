// VALIDATION: compile-fail

class type_info;

int main()
{
  return &typeid(int) ? 0 : 1;
}
