struct value
{
  int member;
};

int main()
{
  value object = {0};
  value * pointer = &object;
  return dynamic_cast<value *>(pointer) == pointer ? 0 : 1;
}
