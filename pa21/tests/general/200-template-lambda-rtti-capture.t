namespace std {
class type_info;
}

template<class T>
int has_lambda_rtti(T value)
{
  auto closure = [value]() { return value; };
  return &typeid(closure) == &typeid(closure);
}

int main()
{
  return has_lambda_rtti(1) ? 0 : 1;
}
