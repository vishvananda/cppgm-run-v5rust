namespace aggregate_appertainment_direct_copy_reference_members {

struct TerminalChar {
  char value;
};

struct TerminalRef {
  char const& value;
};

struct BitwiseOrExpr {
  TerminalChar const& left;
  TerminalRef const& right;
};

struct EndMatcher {
  int value;
};

struct TerminalEnd {
  EndMatcher value;
};

struct ShiftRightExpr {
  BitwiseOrExpr left;
  TerminalEnd right;
};

BitwiseOrExpr const& get_right(BitwiseOrExpr const& expr)
{
  return expr;
}

ShiftRightExpr make_expr(BitwiseOrExpr const& expr, int mark)
{
  EndMatcher end = {mark};
  ShiftRightExpr that = {get_right(expr), {end}};
  return that;
}

int check()
{
  char c = '+';
  TerminalChar left = {'+'};
  TerminalRef right = {c};
  BitwiseOrExpr expr = {left, right};
  ShiftRightExpr result = make_expr(expr, 7);
  if(&result.left.left != &left) {
    return 1;
  }
  if(&result.left.right != &right) {
    return 2;
  }
  if(result.right.value.value != 7) {
    return 3;
  }
  return 0;
}

}

int main()
{
  return aggregate_appertainment_direct_copy_reference_members::check();
}
