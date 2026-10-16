// N3485 8.3.5p5, 15.4p2: a parameter object keeps its declared const
// qualification when a dependent noexcept operand is instantiated; expect reject
template<class T>
int f(const T value) noexcept(noexcept(++value))
{
  return value;
}

int main()
{
  return f(1);
}
