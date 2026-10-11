struct left_owner
{
  static int value;
};

struct right_owner
{
  static int value;
};

int left_owner::value = 3;
int right_owner::value = 5;

template<int *Pointer>
struct pointer_holder
{
  static int read()
  {
    return *Pointer;
  }
};

template<int &Reference>
struct reference_holder
{
  static int read()
  {
    return Reference;
  }
};

int main()
{
  return pointer_holder<&left_owner::value>::read() == 3 &&
         pointer_holder<&right_owner::value>::read() == 5 &&
         reference_holder<left_owner::value>::read() == 3 &&
         reference_holder<right_owner::value>::read() == 5 ? 0 : 1;
}
