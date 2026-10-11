class member_pointer_null_compare_class
{
public:
  int value;

  int method(int arg);
};

int member_pointer_null_compare_class::method(int arg)
{
  return arg + 1;
}

int main()
{
  int member_pointer_null_compare_class::*data_ptr = 0;
  int (member_pointer_null_compare_class::*function_ptr)(int) = 0;

  if(data_ptr != 0) {
    return 1;
  }
  return function_ptr == 0 ? 0 : 2;
}
