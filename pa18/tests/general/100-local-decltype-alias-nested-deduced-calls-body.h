int main()
{
  void (*missing_result)();
  typedef void (*tag)(int);
  MAKE_WRAPPED_TYPE(wrapped_type);
  typedef wrapped_type::type result_type;
  return 0;
}
