#pragma once

#include <cstddef>
#include <string>

int hex_to_value(int c);

char value_to_hex(int c);

std::string hex_dump(const void* pdata, std::size_t nbytes);

std::u32string decode_escape(const std::u32string& data);

std::u32string decode_utf8(const std::string& data);

std::string encode_utf8(const std::u32string& data);

void append_utf8_bytes(char32_t value, std::string& result);

int utf8_tail_length(unsigned char byte);

std::u16string encode_utf16(const std::u32string& data);
