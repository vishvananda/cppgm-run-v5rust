struct Member { Member(const Member &); };
struct Value {
    Member member;
    constexpr Value(const Value &) = default;
};
