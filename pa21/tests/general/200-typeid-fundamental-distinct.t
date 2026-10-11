namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

int main()
{
  if(typeid(int) == typeid(double))
  {
    return 1;
  }
  if(typeid(int) == typeid(int))
  {
    return 0;
  }
  return 2;
}
