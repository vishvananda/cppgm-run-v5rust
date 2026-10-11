int main()
{
  using array_type = bool[];
  return array_type{ true }[0] ? 0 : 1;
}
