// A member function body is a complete-class context, so a member type
// declared later in the class is visible in it (N3485 3.3.7/1).
struct holder
{
  int from_typedef() { later value = 1; return value; }
  int from_alias() { aliased value = 2; return value; }
  int from_nested() { nested value; return value.field; }
  int from_second_declarator() { second value = 3; return value; }

  typedef int later, second;
  using aliased = int;
  struct nested { int field; };
};

int main() { return holder().from_typedef() - 1; }
