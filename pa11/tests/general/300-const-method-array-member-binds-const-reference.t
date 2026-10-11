namespace const_method_array_member_reference
{
  typedef char matrix[3][4];

  unsigned take(const matrix & value, const char *)
  {
    return 0u;
  }

  struct holder
  {
    matrix member;

    unsigned check() const
    {
      return take(member, "member");
    }
  };
}

int main()
{
  const_method_array_member_reference::holder h;
  return h.check();
}
