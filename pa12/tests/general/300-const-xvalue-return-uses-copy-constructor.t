struct item
{
  int state;

  item() : state(7) {}
  item(const item & other) : state(other.state) {}
  item(item && other) : state(other.state) { other.state = 0; }
};

const item && as_const_xvalue(const item & value)
{
  return static_cast<const item &&>(value);
}

item take(const item (& values)[1])
{
  return as_const_xvalue(values[0]);
}

int main()
{
  const item values[] = {item()};
  item result = take(values);
  return result.state == 7 && values[0].state == 7 ? 0 : 1;
}
