struct First {
  int value;
  First(int v) : value(v) {}
};

struct Second {
  int value;
  Second() : value(7) {}
};

struct Tok {
  int prefix;
  First first;
  Second second;
};

int main()
{
  Tok tok{1, 2};
  if(tok.prefix != 1) {
    return 10 + tok.prefix;
  }
  if(tok.first.value != 2) {
    return 20 + tok.first.value;
  }
  if(tok.second.value != 7) {
    return 30 + tok.second.value;
  }
  return 0;
}
