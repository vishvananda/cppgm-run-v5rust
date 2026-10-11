template<class T>
int extern_template_add(T x)
{
  return static_cast<int>(x) + 1;
}

extern template int extern_template_add<int>(int);
