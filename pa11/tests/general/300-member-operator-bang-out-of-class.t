struct Box {
  bool operator!() const;
};
bool Box::operator!() const
{
  return true;
}
int main()
{
  Box box;
  return !box ? 0 : 1;
}
