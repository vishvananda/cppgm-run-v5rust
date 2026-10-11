// VALIDATION: compile-fail
// Throwing a class object initializes an exception object, so the destructor
// for that exception object must be accessible.

struct blocked
{
protected:
  ~blocked() {}
};

void test(blocked &x)
{
  throw x;
}

int main()
{
  return 0;
}
