// VALIDATION: compile-fail

namespace std {
class type_info;
}

int main()
{
  return typeid(int) == typeid(int);
}
