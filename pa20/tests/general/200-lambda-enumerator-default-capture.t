enum ETokenType { OP_LPAREN = 77 };

struct Token {
  bool is_simple(ETokenType type) const;
};

bool f(const Token& tok) {
  const auto check = [&]() -> bool {
    return tok.is_simple(OP_LPAREN);
  };
  return check();
}
