// VALIDATION: compile-fail
// An unused function template must still reject an unknown non-dependent type
// in its trailing return; a parameter pack does not defer that lookup.

template<class... T>
auto skipped(T...) -> missing_type;

int main()
{
  return 0;
}
