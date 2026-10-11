struct Base
{
  virtual int destroy() = 0;

protected:
  ~Base() {}
};

struct Slot
{
  Base **handler;
  void clear();
};

void Slot::clear()
{
  if(handler != 0 && *handler != 0) {
    (*handler)->destroy();
  }
}

int main()
{
  return 0;
}
