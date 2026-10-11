int id(int value) { return value; }

template<class... T>
int f(T... value)
{
  int result(id(value)...);
  return result;
}

int main() { return f(3) - 3; }
