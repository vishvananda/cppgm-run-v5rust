namespace array_const_reference_binding
{
  typedef char matrix[3][4];

  unsigned take(const matrix & value, const char *)
  {
    return 0u;
  }
}

int main()
{
  array_const_reference_binding::matrix local;
  return array_const_reference_binding::take(local, "local");
}
