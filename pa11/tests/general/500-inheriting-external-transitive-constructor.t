struct LibraryBase {
  struct Token {
    int value;
  };

  LibraryBase(const Token &token);
};

struct Soft : LibraryBase {
  using LibraryBase::LibraryBase;
};

struct Hard : Soft {
  using Soft::Soft;
};

int main() {
  LibraryBase::Token token = {7};
  Soft soft(token);
  Hard hard(token);
  return 0;
}
