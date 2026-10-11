// VALIDATION: compile-fail
// A dependent argument does not make an unknown template name dependent.

template<class T, class... Ts>
auto skipped(T, Ts...) -> missing_template<T>;

int main()
{
  return 0;
}
