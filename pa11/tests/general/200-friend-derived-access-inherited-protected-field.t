class friend_inherited_base {
protected:
  int value;
public:
  friend_inherited_base(int v) : value(v) {}
};

class friend_inherited_reader;

class friend_inherited_property : private friend_inherited_base {
  friend class friend_inherited_reader;
public:
  friend_inherited_property(int v) : friend_inherited_base(v) {}
};

class friend_inherited_reader {
public:
  int read(friend_inherited_property & p)
  {
    ++p.value;
    return p.value;
  }
};

int main()
{
  friend_inherited_property p(4);
  friend_inherited_reader reader;
  return reader.read(p) == 5 ? 0 : 1;
}
