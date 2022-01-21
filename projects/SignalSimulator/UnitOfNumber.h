#ifndef UNIT_OF_NUMBER_H
#define UNIT_OF_NUMBER_H

#include <stdint.h>
#include <type_traits>
#include <cmath>
#include <cassert>

namespace UnitOfNumber {

	/// <summary>
	/// 基本数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseNumeral
	{
	private:
		__ValueType _Value;

	protected:
		inline __ValueType GetValue() const noexcept
		{
			return _Value;
		}
		inline void SetValue(const __ValueType&& v) & noexcept
		{
			_Value = v;
		}

		template<class T>
		constexpr auto CAST(T&& v) { return static_cast<__ValueType>(v); }

		template<class T>
		constexpr T mod(T lhs, T rhs) noexcept
		{
			if constexpr (std::is_integral<T>::value)
			{
				return lhs % rhs;
			}
			else if constexpr (std::is_floating_point<T>::value)
			{
				return std::fmod(lhs, rhs);
			}
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseNumeral() noexcept = delete;
		//BaseNumeral(const BaseNumeral&) noexcept = delete;
		//BaseNumeral(BaseNumeral&&) noexcept = delete;
		//~BaseNumeral() noexcept = default;
		//**********************************************************
		constexpr BaseNumeral(const __ValueType& init) noexcept : _Value(init)
		{}
		constexpr BaseNumeral(const __ValueType&& init = 0) noexcept : BaseNumeral(init)
		{}
		constexpr BaseNumeral(const BaseNumeral & other) noexcept : BaseNumeral(other.GetValue())
		{}
		constexpr BaseNumeral(const BaseNumeral && other) noexcept : BaseNumeral(other)
		{}
		constexpr ~BaseNumeral() noexcept
		{}

		//キャスト演算子(Cast)
		inline explicit operator __ValueType() const noexcept
		{
			return GetValue();
		}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//BaseNumeral& operator=(const BaseNumeral&) noexcept = delete;
		//BaseNumeral& operator=(BaseNumeral&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline BaseNumeral& operator=(T& rhs) noexcept
		{
			SetValue(rhs);
			return *this;
		}
		template<> inline BaseNumeral& operator=(BaseNumeral& rhs) noexcept
		{
			SetValue(rhs.GetValue());
			return *this;
		}
		template<class T> inline BaseNumeral& operator=(T&& rhs) & noexcept
		{
			SetValue(rhs);
			return *this;
		}
		template<> inline BaseNumeral& operator=(BaseNumeral&& rhs) & noexcept
		{
			SetValue(rhs.GetValue());
			return *this;
		}

		//単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		inline BaseNumeral operator+() const { return BaseNumeral(+GetValue()); }
		inline BaseNumeral operator-() const { return BaseNumeral(-GetValue()); }

		//算術演算子(Arithmetic)
		template<class T> inline BaseNumeral operator+(T&& rhs) { return BaseNumeral(GetValue() + CAST(rhs)); }
		template<class T> inline BaseNumeral operator-(T&& rhs) { return BaseNumeral(GetValue() - CAST(rhs)); }
		template<class T> inline BaseNumeral operator*(T&& rhs) { return BaseNumeral(GetValue() * CAST(rhs)); }
		template<class T> inline BaseNumeral operator/(T&& rhs) { return BaseNumeral(GetValue() / CAST(rhs)); }
		template<class T> inline BaseNumeral operator%(T&& rhs) { return BaseNumeral(mod(GetValue(), CAST(rhs))); }

		//複合代入演算子(Compound Assignment)
		template<class T> inline void operator+=(T&& rhs) { SetValue(GetValue() + rhs); }
		template<class T> inline void operator-=(T&& rhs) { SetValue(GetValue() - rhs); }
		template<class T> inline void operator*=(T&& rhs) { SetValue(GetValue() * rhs); }
		template<class T> inline void operator/=(T&& rhs) { SetValue(GetValue() / rhs); }
		template<class T> inline void operator%=(T&& rhs) { SetValue(GetValue() % rhs); }
		template<>        inline void operator+=(BaseNumeral& rhs) { SetValue(GetValue() + rhs.GetValue()); }
		template<>        inline void operator-=(BaseNumeral& rhs) { SetValue(GetValue() - rhs.GetValue()); }
		template<>        inline void operator*=(BaseNumeral& rhs) { SetValue(GetValue() * rhs.GetValue()); }
		template<>        inline void operator/=(BaseNumeral& rhs) { SetValue(GetValue() / rhs.GetValue()); }
		template<>        inline void operator%=(BaseNumeral& rhs) { SetValue(mod(GetValue(), rhs.GetValue())); }

		//後置インクリメント/デクリメント(Postfix Increment/Decrement)
		inline BaseNumeral operator++(int) { auto z1 = GetValue(); SetValue(z1 + 1); return BaseNumeral(z1); }
		inline BaseNumeral operator--(int) { auto z1 = GetValue(); SetValue(z1 - 1); return BaseNumeral(z1); }

		//前置インクリメント/デクリメント(Prefix Increment/Decremrnt)
		inline BaseNumeral& operator++() { SetValue(GetValue() + 1); return *this; }
		inline BaseNumeral& operator--() { SetValue(GetValue() - 1); return *this; }

		//論理否定演算子(Logical Not)
		inline bool operator!() const noexcept { return GetValue() != 0; }

		//比較演算子(Compare)
		template<class T> inline bool operator==(T&& rhs) const { return GetValue() == rhs; }
		template<class T> inline bool operator!=(T&& rhs) const { return GetValue() != rhs; }
		template<class T> inline bool operator<=(T&& rhs) const { return GetValue() <= rhs; }
		template<class T> inline bool operator< (T&& rhs) const { return GetValue() <  rhs; }
		template<class T> inline bool operator> (T&& rhs) const { return GetValue() >  rhs; }
		template<class T> inline bool operator>=(T&& rhs) const { return GetValue() >= rhs; }
		template<>        inline bool operator==(BaseNumeral& rhs) const { return GetValue() == rhs.GetValue(); }
		template<>        inline bool operator!=(BaseNumeral& rhs) const { return GetValue() != rhs.GetValue(); }
		template<>        inline bool operator<=(BaseNumeral& rhs) const { return GetValue() <= rhs.GetValue(); }
		template<>        inline bool operator< (BaseNumeral& rhs) const { return GetValue() <  rhs.GetValue(); }
		template<>        inline bool operator> (BaseNumeral& rhs) const { return GetValue() >  rhs.GetValue(); }
		template<>        inline bool operator>=(BaseNumeral& rhs) const { return GetValue() >= rhs.GetValue(); }

		//科学算術
		template<class T> inline BaseNumeral pow(T&& rhs) { return BaseNumeral(std::pow(GetValue(), rhs)); }
		template<>        inline BaseNumeral pow(BaseNumeral& rhs) { return BaseNumeral(std::pow(GetValue(), rhs.GetValue())); }
						  inline BaseNumeral log() { return BaseNumeral(std::log(GetValue())); }
		template<class T> inline BaseNumeral log(T&& rhs) { return BaseNumeral(std::log(GetValue() / rhs)); }
		template<>        inline BaseNumeral log(BaseNumeral& rhs) { return BaseNumeral(std::log(GetValue() / rhs.log())); }
						  inline BaseNumeral abs() { return BaseNumeral(std::abs(GetValue())); }
	};

	/// <summary>
	/// 数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	using Numeral = BaseNumeral<__ValueType>;
	namespace {
		using NumeralValueType = long double;
		static_assert(sizeof(Numeral<NumeralValueType>) == sizeof(NumeralValueType), "Numeral Size Error");
	}

	/// <summary>
	/// 基本固定小数点数クラス
	/// </summary>
	/// <typeparam name="__MantissaType"></typeparam>
	/// <typeparam name="__ExponentType"></typeparam>
	template<class __MantissaType, class __ExponentType>
	class BaseFixedPointNumber
	{
		static_assert(
			std::is_arithmetic<__MantissaType>::value,
			"Mantissa can only be an arithmetic type."
			);
		static_assert(
			std::is_signed<__ExponentType>::value && std::is_integral<__ExponentType>::value,
			"Exponentiation can only be specified for integer and signed arithmetic types."
		);

	private:
		std::shared_ptr<__MantissaType> _pMantissa;
		std::shared_ptr<__ExponentType> _pExponent;

	protected:
		inline __MantissaType GetMantissa() const noexcept
		{
			return *_pMantissa;
		}
		inline void SetMantissa(const __MantissaType&& v) & noexcept
		{
			*_pMantissa = v;
		}
		inline __ExponentType GetExponent() const noexcept
		{
			return *_pExponent;
		}
		inline void SetExponent(const __ExponentType&& v) & noexcept
		{
			*_pExponent = v;
		}

		template<class T>
		constexpr auto M_CAST(T&& v) { return static_cast<__MantissaType>(v); }

		template<class T>
		constexpr auto E_CAST(T&& v) { return static_cast<__ExponentType>(v); }

		constexpr BaseFixedPointNumber(const BaseFixedPointNumber& other) noexcept
		{
			_pMantissa = other._pMantissa;
			_pExponent = other._pExponent;
		}

		template<class T>
		constexpr T mod(T lhs, T rhs) noexcept
		{
			if constexpr (std::is_integral<T>::value)
			{
				return lhs % rhs;
			}
			else if constexpr (std::is_floating_point<T>::value)
			{
				return std::fmod(lhs, rhs);
			}
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseFixedPointNumber() noexcept = delete;
		//BaseFixedPointNumber(const BaseFixedPointNumber&) = delete;
		BaseFixedPointNumber(BaseFixedPointNumber&&) noexcept = delete;
		~BaseFixedPointNumber() noexcept = default;
		//**********************************************************
		constexpr BaseFixedPointNumber(const __MantissaType& m_init, const __ExponentType& e_init) noexcept 
		: _pMantissa(std::make_shared<__MantissaType>(m_init))
		, _pExponent(std::make_shared<__ExponentType>(e_init))
		{}
		constexpr BaseFixedPointNumber(const __MantissaType && m_init, const __ExponentType && e_init) noexcept
		: BaseFixedPointNumber(m_init, e_init)
		{}

		//キャスト演算子(Cast)
		//inline explicit operator __ValueType() const noexcept
		//{
		//	return GetValue();
		//}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		BaseFixedPointNumber& operator=(const BaseFixedPointNumber&) = delete;
		//BaseFixedPointNumber& operator=(BaseFixedPointNumber&&) noexcept = delete;
		//**********************************************************
		inline BaseFixedPointNumber& operator=(BaseFixedPointNumber&& rhs) & noexcept
		{
			SetMantissa(rhs.SetMantissa());
			SetExponent(rhs.GetExponent());
			return *this;
		}

		////単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		//inline BaseNumeral operator+() const { return BaseNumeral(+GetValue()); }
		//inline BaseNumeral operator-() const { return BaseNumeral(-GetValue()); }

		////算術演算子(Arithmetic)
		//template<class T> inline BaseNumeral operator+(T&& rhs) { return BaseNumeral(GetValue() + CAST(rhs)); }
		//template<class T> inline BaseNumeral operator-(T&& rhs) { return BaseNumeral(GetValue() - CAST(rhs)); }
		//template<class T> inline BaseNumeral operator*(T&& rhs) { return BaseNumeral(GetValue() * CAST(rhs)); }
		//template<class T> inline BaseNumeral operator/(T&& rhs) { return BaseNumeral(GetValue() / CAST(rhs)); }
		//template<class T> inline BaseNumeral operator%(T&& rhs) { return BaseNumeral(mod(GetValue(), CAST(rhs))); }

		////複合代入演算子(Compound Assignment)
		//template<class T> inline void operator+=(T&& rhs) { SetValue(GetValue() + rhs); }
		//template<class T> inline void operator-=(T&& rhs) { SetValue(GetValue() - rhs); }
		//template<class T> inline void operator*=(T&& rhs) { SetValue(GetValue() * rhs); }
		//template<class T> inline void operator/=(T&& rhs) { SetValue(GetValue() / rhs); }
		//template<class T> inline void operator%=(T&& rhs) { SetValue(GetValue() % rhs); }
		//template<>        inline void operator+=(BaseNumeral& rhs) { SetValue(GetValue() + rhs.GetValue()); }
		//template<>        inline void operator-=(BaseNumeral& rhs) { SetValue(GetValue() - rhs.GetValue()); }
		//template<>        inline void operator*=(BaseNumeral& rhs) { SetValue(GetValue() * rhs.GetValue()); }
		//template<>        inline void operator/=(BaseNumeral& rhs) { SetValue(GetValue() / rhs.GetValue()); }
		//template<>        inline void operator%=(BaseNumeral& rhs) { SetValue(mod(GetValue(), rhs.GetValue())); }

		////後置インクリメント/デクリメント(Postfix Increment/Decrement)
		//inline BaseNumeral operator++(int) { auto z1 = GetValue(); SetValue(z1 + 1); return BaseNumeral(z1); }
		//inline BaseNumeral operator--(int) { auto z1 = GetValue(); SetValue(z1 - 1); return BaseNumeral(z1); }

		////前置インクリメント/デクリメント(Prefix Increment/Decremrnt)
		//inline BaseNumeral& operator++() { SetValue(GetValue() + 1); return *this; }
		//inline BaseNumeral& operator--() { SetValue(GetValue() - 1); return *this; }

		////論理否定演算子(Logical Not)
		//inline bool operator!() const noexcept { return GetValue() != 0; }

		////比較演算子(Compare)
		//template<class T> inline bool operator==(T&& rhs) const { return GetValue() == rhs; }
		//template<class T> inline bool operator!=(T&& rhs) const { return GetValue() != rhs; }
		//template<class T> inline bool operator<=(T&& rhs) const { return GetValue() <= rhs; }
		//template<class T> inline bool operator< (T&& rhs) const { return GetValue() <  rhs; }
		//template<class T> inline bool operator> (T&& rhs) const { return GetValue() >  rhs; }
		//template<class T> inline bool operator>=(T&& rhs) const { return GetValue() >= rhs; }
		//template<>        inline bool operator==(BaseNumeral& rhs) const { return GetValue() == rhs.GetValue(); }
		//template<>        inline bool operator!=(BaseNumeral& rhs) const { return GetValue() != rhs.GetValue(); }
		//template<>        inline bool operator<=(BaseNumeral& rhs) const { return GetValue() <= rhs.GetValue(); }
		//template<>        inline bool operator< (BaseNumeral& rhs) const { return GetValue() <  rhs.GetValue(); }
		//template<>        inline bool operator> (BaseNumeral& rhs) const { return GetValue() >  rhs.GetValue(); }
		//template<>        inline bool operator>=(BaseNumeral& rhs) const { return GetValue() >= rhs.GetValue(); }

		////科学算術
		//template<class T> inline BaseNumeral pow(T&& rhs) { return BaseNumeral(std::pow(GetValue(), rhs)); }
		//template<>        inline BaseNumeral pow(BaseNumeral& rhs) { return BaseNumeral(std::pow(GetValue(), rhs.GetValue())); }
		//				  inline BaseNumeral log() { return BaseNumeral(std::log(GetValue())); }
		//template<class T> inline BaseNumeral log(T&& rhs) { return BaseNumeral(std::log(GetValue() / rhs)); }
		//template<>        inline BaseNumeral log(BaseNumeral& rhs) { return BaseNumeral(std::log(GetValue() / rhs.log())); }
		//				  inline BaseNumeral abs() { return BaseNumeral(std::abs(GetValue())); }
	};

	///// <summary>
	///// 数字クラス
	///// </summary>
	///// <typeparam name="__ValueType"></typeparam>
	//template<class __ValueType>
	//using Numeral = BaseNumeral<__ValueType>;
	//namespace {
	//	using NumeralValueType = long double;
	//	static_assert(sizeof(Numeral<NumeralValueType>) == sizeof(std::shared_ptr<NumeralValueType>), "Numeral Size Error");
	//}

	///// <summary>
	///// 基本SI接頭辞単位クラス
	///// </summary>
	///// <typeparam name="__ValueType"></typeparam>
	///// <typeparam name="__Exp"></typeparam>
	template<class __ValueType, int __Exp = 0>
	class BaseSIPrefixUnit : public BaseNumeral<__ValueType>
	{
	protected:
		//元の値を指定の単位へ
		template<class T> inline __ValueType OriginalToSpecific(T&& v) const noexcept
		{
			return static_cast<__ValueType>(v * std::pow(10, -__Exp));
		}

		//指定の値を元の値へ
		template<class T> inline __ValueType SpecificToOriginal(T&& v) & noexcept
		{
			return static_cast<__ValueType>(v * std::pow(10, __Exp));
		}
	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseSIPrefixUnit() noexcept = delete;
		//BaseSIPrefixUnit(const BaseSIPrefixUnit&) = delete;
		//BaseSIPrefixUnit(BaseSIPrefixUnit&&) noexcept = delete;
		~BaseSIPrefixUnit() noexcept = default;
		//**********************************************************
		constexpr BaseSIPrefixUnit(const __ValueType&& init = 0) noexcept : BaseNumeral<__ValueType>(std::move(init))
		{}
		constexpr BaseSIPrefixUnit(const BaseNumeral<__ValueType>& other) noexcept : BaseNumeral<__ValueType>(other)
		{}

		//キャスト演算子(Cast)
		explicit operator __ValueType() const noexcept
		{			
			return OriginalToSpecific(this->GetValue());
		}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		BaseSIPrefixUnit& operator=(const BaseSIPrefixUnit&) = delete;
		//BaseSIPrefixUnit& operator=(BaseSIPrefixUnit&&) noexcept = delete;
		//**********************************************************
		template<class T> inline BaseSIPrefixUnit& operator=(T&& rhs) & noexcept
		{
			this->SetValue(SpecificToOriginal(rhs));
			return *this;
		}
		template<> inline BaseSIPrefixUnit& operator=(BaseSIPrefixUnit&& rhs) & noexcept
		{
			this->SetValue(SpecificToOriginal(rhs.GetValue()));
			return *this;
		}
	};

	/// <summary>
	/// 基本SI接頭辞テンプレートクラス
	/// 型を指定できる
	/// ※小数点のある型が望ましい
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseSIPrefix : public BaseSIPrefixUnit<__ValueType>
	{
	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseSIPrefix() noexcept = delete;
		BaseSIPrefix(const BaseSIPrefix&) = delete;
		BaseSIPrefix(BaseSIPrefix&&) noexcept = delete;
		~BaseSIPrefix() noexcept = default;
		//**********************************************************
		constexpr BaseSIPrefix(const __ValueType&& init = 0) noexcept : BaseSIPrefixUnit<__ValueType>(std::move(init))
		, Q(*this)
		, R(*this)
		, Y(*this)
		, Z(*this)
		, E(*this)
		, P(*this)
		, T(*this)
		, G(*this)
		, M(*this)
		, k(*this)
		, h(*this)
		, da(*this)
		, base(*this)
		, d(*this)
		, c(*this)
		, m(*this)
		, u(*this)
		, n(*this)
		, p(*this)
		, f(*this)
		, a(*this)
		, z(*this)
		, y(*this)
		, r(*this)
		, q(*this)
		{}

		BaseSIPrefixUnit<__ValueType,  30> Q;
		BaseSIPrefixUnit<__ValueType,  27> R;
		BaseSIPrefixUnit<__ValueType,  24> Y;
		BaseSIPrefixUnit<__ValueType,  21> Z;
		BaseSIPrefixUnit<__ValueType,  18> E;
		BaseSIPrefixUnit<__ValueType,  15> P;
		BaseSIPrefixUnit<__ValueType,  12> T;
		BaseSIPrefixUnit<__ValueType,   9> G;
		BaseSIPrefixUnit<__ValueType,   6> M;
		BaseSIPrefixUnit<__ValueType,   3> k;
		BaseSIPrefixUnit<__ValueType,   2> h;
		BaseSIPrefixUnit<__ValueType,   1> da;
		BaseSIPrefix&					   base;
		BaseSIPrefixUnit<__ValueType, - 1> d;
		BaseSIPrefixUnit<__ValueType, - 2> c;
		BaseSIPrefixUnit<__ValueType, - 3> m;
		BaseSIPrefixUnit<__ValueType, - 6> u;
		BaseSIPrefixUnit<__ValueType, - 9> n;
		BaseSIPrefixUnit<__ValueType, -12> p;
		BaseSIPrefixUnit<__ValueType, -15> f;
		BaseSIPrefixUnit<__ValueType, -18> a;
		BaseSIPrefixUnit<__ValueType, -21> z;
		BaseSIPrefixUnit<__ValueType, -24> y;
		BaseSIPrefixUnit<__ValueType, -27> r;
		BaseSIPrefixUnit<__ValueType, -30> q;

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		BaseSIPrefix& operator=(const BaseSIPrefix&) = delete;
		BaseSIPrefix& operator=(BaseSIPrefix&&) noexcept = delete;
		//**********************************************************
		template<class _T> inline BaseSIPrefixUnit<__ValueType>& operator=(_T&& rhs) & noexcept
		{
			Q  = rhs;
			R  = rhs;
			Y  = rhs;
			Z  = rhs;
			E  = rhs;
			P  = rhs;
			T  = rhs;
			G  = rhs;
			M  = rhs;
			k  = rhs;
			h  = rhs;
			da = rhs;
			BaseSIPrefixUnit<__ValueType>::operator=(rhs);
			d  = rhs;
			c  = rhs;
			m  = rhs;
			u  = rhs;
			n  = rhs;
			p  = rhs;
			f  = rhs;
			a  = rhs;
			z  = rhs;
			y  = rhs;
			r  = rhs;
			q  = rhs;
			return *this;
		}
		template<> inline BaseSIPrefixUnit<__ValueType>& operator=(BaseSIPrefix&& rhs) & noexcept
		{
			Q  = rhs;
			R  = rhs;
			Y  = rhs;
			Z  = rhs;
			E  = rhs;
			P  = rhs;
			T  = rhs;
			G  = rhs;
			M  = rhs;
			k  = rhs;
			h  = rhs;
			da = rhs;
			BaseSIPrefixUnit<__ValueType>::operator=(rhs);
			d  = rhs;
			c  = rhs;
			m  = rhs;
			u  = rhs;
			n  = rhs;
			p  = rhs;
			f  = rhs;
			a  = rhs;
			z  = rhs;
			y  = rhs;
			r  = rhs;
			q  = rhs;
			return *this;
		}
	};

	/// <summary>
	/// SI接頭辞クラス
	/// </summary>
	template<class __ValueType>
	using SIPrefix = BaseSIPrefix<__ValueType>;
	namespace {
		using SIPrefixType = long double;
		static_assert(sizeof(SIPrefix<SIPrefixType>) == sizeof(SIPrefixType) * 26, "SIPrefix Size Error");
	}
}

#endif // UNIT_OF_NUMBER_H
