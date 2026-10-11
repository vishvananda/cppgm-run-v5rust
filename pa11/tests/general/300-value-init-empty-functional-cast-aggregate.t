struct message
{
  void * name;
  unsigned name_length;
  void * control;
  int flags;
};

int main()
{
  message value = message();
  return value.name == 0 &&
         value.name_length == 0 &&
         value.control == 0 &&
         value.flags == 0 ? 0 : 1;
}
