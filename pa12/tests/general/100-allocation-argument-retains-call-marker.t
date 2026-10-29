// An allocation nested in a reference argument keeps the enclosing call's
// operand boundary after construction releases its deallocation cleanup.
struct Guard { ~Guard(); };
struct Node { int value; Node(const Node&); };
void consume(Node* const&);
void allocation_argument(const Node& source) {
    Guard guard;
    consume(new Node(source));
}
