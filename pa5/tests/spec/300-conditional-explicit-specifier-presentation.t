explicit(false) int value;
explicit(true) int free_function();
struct Box {
    explicit(false) int member_function();
    explicit(true) Box(int);
    explicit(false) operator int() const;
};
