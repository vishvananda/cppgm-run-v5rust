struct YNode {
  YNode *next;
  int value;
};

YNode g;

int main() {
  return sizeof(g);
}
