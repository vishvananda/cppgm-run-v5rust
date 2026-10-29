// expect reject
struct Member {
    Member();
    Member(Member&);
    Member(int);
    operator int() const;
};
struct Owner {
    Member members[9];
    Owner();
    Owner(const Owner&) = default;
};
void consume(const Owner& value) { Owner copy(value); }
int main() { return 0; }
