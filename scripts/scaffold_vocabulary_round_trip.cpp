// The shipped scaffold headers on their own: every one compiles in a single
// translation unit with only the shipped headers on the include path, and
// every spelling of the vocabulary headers reads back as its own value.
// Exits 0 when all hold; prints each failure to standard error.
//
// scripts/export_student_repo.sh builds and runs this against the export
// (-std=gnu++11 and CPPGM_STDLIB_FLAGS), and scripts/tests runs it against
// dev/src.

#include "abi/itanium/abi_mangle.h"
#include "ir_symbol_model.h"
#include "lowir_model.h"
#include "mir_model.h"
#include "native/mir/registers.h"
#include "preprocess/tokens/DebugPPTokenStream.h"
#include "preprocess/tokens/IPPTokenStream.h"
#include "support/not_implemented.h"
#include "support/tool_help_text.h"

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>

namespace {

int failures = 0;

void expect(bool ok, const char* what, unsigned index)
{
	if (!ok)
	{
		std::fprintf(stderr, "vocabulary round trip failed: %s %u\n", what, index);
		++failures;
	}
}

// Enumerators [first, count) of E read back from their spellings.
template<typename E>
void round_trip(const char* what, unsigned count, unsigned first = 0)
{
	for (unsigned i = first; i < count; ++i)
	{
		const char* text = spelling(static_cast<E>(i));
		E back = static_cast<E>(0);
		expect(text != 0 && from_spelling(text, std::strlen(text), &back) && back == static_cast<E>(i), what, i);
	}
}

void check_metadata()
{
	using namespace ir_model;
	// The default value of an optional fact is spelled "" and has no reading.
	round_trip<SymbolRole>("SymbolRole", kSymbolRoleCount, 1);
	round_trip<LanguageLinkageMode>("LanguageLinkageMode", kLanguageLinkageModeCount, 1);
	round_trip<SymbolBindingMode>("SymbolBindingMode", kSymbolBindingModeCount, 1);
	round_trip<GlobalStorageMode>("GlobalStorageMode", kGlobalStorageModeCount, 1);
	round_trip<CallArityMode>("CallArityMode", kCallArityModeCount, 1);
	round_trip<CallEffectsMode>("CallEffectsMode", kCallEffectsModeCount, 1);
	round_trip<CallUnwindMode>("CallUnwindMode", kCallUnwindModeCount, 1);
	round_trip<CallReturnMode>("CallReturnMode", kCallReturnModeCount, 1);
	round_trip<ParamPassingMode>("ParamPassingMode", kParamPassingModeCount, 1);
	round_trip<ParamAliasMode>("ParamAliasMode", kParamAliasModeCount, 1);
	round_trip<IndexProjectionKind>("IndexProjectionKind", kIndexProjectionKindCount, 1);
	round_trip<MetadataKey>("MetadataKey", kMetadataKeyCount);
	// Every stated classification of an 8- or 16-byte object writes a word
	// that reads back; the host-object words only where they are admitted.
	for (unsigned bytes = 8; bytes <= 16; bytes += 8)
	{
		for (unsigned v = 0; v < 256; ++v)
		{
			const std::uint8_t classes = static_cast<std::uint8_t>(v);
			const bool host = classes == (OC_STATED | OC_X87) || classes == (OC_STATED | OC_COMPLEX_X87);
			const bool stated = (classes & ~(OC_STATED | 15)) == 0 && (classes & OC_STATED) != 0 &&
				(bytes == 16 || (classes & 12) == 0);
			if (classes != (OC_STATED | OC_MEMORY) && !host && !stated)
			{
				continue;
			}
			const bool sseup = eightbyte_class(classes, 0) == EB_SSEUP || eightbyte_class(classes, 1) == EB_SSEUP;
			std::string word;
			append_object_classes(word, classes, bytes);
			expect(object_classes_from_spelling(word.data(), word.size(), bytes, true) == classes, "classes", v);
			expect(host || sseup || object_classes_from_spelling(word.data(), word.size(), bytes, false) == classes,
				"course classes", v);
		}
	}
}

void check_lowir()
{
	using namespace lowir_model;
	// `obj` takes its <bytes x align> suffix, so it is not a scalar type word.
	round_trip<TypeKind>("TypeKind", static_cast<unsigned>(TypeKind::Obj));
	round_trip<Opcode>("Opcode", kOpcodeCount);
	round_trip<UnaryOp>("UnaryOp", kUnaryOpCount);
	round_trip<BinaryOp>("BinaryOp", kBinaryOpCount);
	round_trip<ComparePredicate>("ComparePredicate", kComparePredicateCount);
	round_trip<ConvertOp>("ConvertOp", kConvertOpCount);
	round_trip<Keyword>("Keyword", kKeywordCount);
	round_trip<NamedLiteral>("NamedLiteral", kNamedLiteralCount);
	for (unsigned bit = 1; bit < 256; bit <<= 1)
	{
		const char* text = spelling(static_cast<InstructionFlag>(bit));
		InstructionFlag back = IF_VOLATILE;
		expect(text == 0 || (from_spelling(text, std::strlen(text), &back) && back == bit), "InstructionFlag", bit);
	}
	for (unsigned i = 0; i < kValueKindCount; ++i)
	{
		const char s = sigil(static_cast<ValueKind>(i));
		expect(s == 0 || value_kind_of_sigil(s) == static_cast<ValueKind>(i), "ValueKind", i);
	}
	for (unsigned i = 0; i < kComparePredicateCount; ++i)
	{
		const ComparePredicate p = static_cast<ComparePredicate>(i);
		expect(swapped(swapped(p)) == p && inverted(inverted(p)) == p, "ComparePredicate facts", i);
	}
}

void check_mir()
{
	using namespace mir_model;
	for (unsigned r = 0; r < kGprCount + kXmmCount; ++r)
	{
		const char* text = register_spelling(static_cast<Reg>(r));
		Reg back = kNoReg;
		expect(text != 0 && register_from_spelling(text, std::strlen(text), &back) && back == r, "Reg", r);
	}
	round_trip<Cond>("Cond", kCondCount);
	for (unsigned c = 0; c < kCondCount; ++c)
	{
		expect(invert(invert(static_cast<Cond>(c))) == c, "Cond facts", c);
	}
	for (unsigned i = 0; i < kOpCount; ++i)
	{
		expect(spelling(static_cast<Op>(i)) != 0, "Op", i);
	}
	for (unsigned i = 0; i < kConvKindCount; ++i)
	{
		ConvKind back = ConvKind::SIToFP;
		expect(
			conv_kind(convert_op(static_cast<ConvKind>(i)), &back) && back == static_cast<ConvKind>(i), "ConvKind", i);
	}
}

void check_abi()
{
	using namespace abi_mangle;
	for (unsigned i = 1; i < ABI_BUILTIN_TYPE_COUNT; ++i)
	{
		const AbiBuiltinTypeKind kind = static_cast<AbiBuiltinTypeKind>(i);
		const char* word = abi_builtin_type_word(kind);
		expect(abi_builtin_type_kind(word, std::strlen(word)) == kind, "AbiBuiltinTypeKind", i);
	}
	for (unsigned i = 1; i < ABI_STANDARD_SUBSTITUTION_COUNT; ++i)
	{
		const AbiStandardSubstitutionKind kind = static_cast<AbiStandardSubstitutionKind>(i);
		const char* code = abi_standard_substitution_code(kind);
		expect(abi_standard_substitution_kind(code, std::strlen(code)) == kind, "AbiStandardSubstitutionKind", i);
	}
	for (unsigned i = 1; i < ABI_TERMINAL_COUNT; ++i)
	{
		const char* word = abi_terminal_word(static_cast<AbiTerminalKind>(i));
		AbiTerminalKind back = ABI_TERMINAL_NONE;
		expect(abi_find_terminal_kind(word, std::strlen(word), &back) && back == i, "AbiTerminalKind", i);
	}
	std::size_t operations = 0;
	expect(abi_operation_entries(&operations) != 0 && operations != 0, "AbiOperationEntry", 0);
	round_trip<AbiFactDefinition>("AbiFactDefinition", ABI_FACT_DEFINITION_COUNT);
	round_trip<AbiFactTarget>("AbiFactTarget", ABI_FACT_TARGET_COUNT);
	round_trip<AbiFactFunctionForm>("AbiFactFunctionForm", ABI_FACT_FUNCTION_COUNT);
	round_trip<AbiFactRecord>("AbiFactRecord", ABI_FACT_RECORD_COUNT);
	round_trip<AbiFactQualifier>("AbiFactQualifier", ABI_FACT_QUALIFIER_COUNT);
	round_trip<AbiFactSpecialTerminal>("AbiFactSpecialTerminal", ABI_FACT_TERMINAL_COUNT);
	round_trip<AbiFactTypeForm>("AbiFactTypeForm", ABI_FACT_TYPE_COUNT);
	round_trip<AbiFactArgumentForm>("AbiFactArgumentForm", ABI_FACT_ARGUMENT_COUNT);
	round_trip<AbiFactExpressionForm>("AbiFactExpressionForm", ABI_FACT_EXPRESSION_COUNT);
	round_trip<AbiFactContextForm>("AbiFactContextForm", ABI_FACT_CONTEXT_COUNT);
	round_trip<AbiFactEntityForm>("AbiFactEntityForm", ABI_FACT_ENTITY_COUNT);
	round_trip<AbiFactWord>("AbiFactWord", ABI_FACT_WORD_COUNT);
	for (unsigned i = 0; i < ABI_FACT_PREFIX_COUNT; ++i)
	{
		const char* text = spelling(static_cast<AbiFactTypePrefix>(i));
		AbiFactTypePrefix back = ABI_FACT_PREFIX_PTR;
		expect(text != 0 && prefix_from_spelling(text, std::strlen(text), &back) && back == i, "AbiFactTypePrefix", i);
	}
}

} // namespace

int main()
{
	check_metadata();
	check_lowir();
	check_mir();
	check_abi();
	expect(CPPGM_EXIT_NOT_IMPLEMENTED == 86, "CPPGM_EXIT_NOT_IMPLEMENTED", 0);
	return failures == 0 ? 0 : 1;
}
