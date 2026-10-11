namespace semantic_conversion {
struct ExprInfo {
  ExprInfo & operator=(const ExprInfo & other);
  ExprInfo & operator=(ExprInfo && other);
};

ExprInfo & ExprInfo::operator=(const ExprInfo & other) {
  (void)other;
  return *this;
}

ExprInfo & ExprInfo::operator=(ExprInfo && other) {
  (void)other;
  return *this;
}
}

int main() {
  semantic_conversion::ExprInfo a;
  semantic_conversion::ExprInfo b;
  a = static_cast<semantic_conversion::ExprInfo &&>(b);
  return 0;
}
