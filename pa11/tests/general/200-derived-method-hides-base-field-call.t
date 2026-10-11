enum YOp { OpMatch };

struct YBase {
  YOp opcode;
  YBase(YOp op) : opcode(op) {}
};

struct YDerived : public YBase {
  YDerived() : YBase(OpMatch) {}

  YOp opcode() {
    return YBase::opcode;
  }

  int matches() {
    if(opcode() == OpMatch) {
      return 7;
    }
    return 3;
  }
};

int main() {
  YDerived x;
  return x.matches();
}
