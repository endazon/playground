#pragma once

#include <stdint.h>
#include <type_traits>
#include <cmath>
#include <cassert>

namespace UnitOfNumber {

#pragma region Numeral
	/// <summary>
	/// 数字実体クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class NumericEntity
	{
	private:
		using __MySelfType = NumericEntity;
		__ValueType _Value;

	protected:
		template<class T>
		constexpr auto CAST(T&& v) const { return static_cast<__ValueType>(v); }

		inline __ValueType GetValue() const noexcept
		{
			return _Value;
		}
		inline void SetValue(const __ValueType& v) & noexcept
		{
			_Value = v;
		}
		inline void SetValue(const __ValueType&& v) & noexcept
		{
			SetValue(v);
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		//NumericEntity() noexcept = delete;
		//NumericEntity(const __MySelfType&) noexcept = delete;
		//NumericEntity(__MySelfType&&) noexcept = delete;
		constexpr ~NumericEntity() noexcept = default;
		//**********************************************************
		constexpr NumericEntity(const __ValueType& init) noexcept : _Value(init)
		{}
		constexpr NumericEntity(const __ValueType&& init = 0) noexcept : __MySelfType(init)
		{}
		template<class T> constexpr NumericEntity(const T& other) noexcept : __MySelfType(CAST(other))
		{}
		template<class T> constexpr NumericEntity(const T&& other) noexcept : __MySelfType(other)
		{}

		//キャスト演算子(Cast)
		inline explicit operator __ValueType() const noexcept
		{
			return GetValue();
		}
	};

	/// <summary>
	/// 数字操作クラス
	/// </summary>
	/// <typeparam name="__InheritanceType"></typeparam>
	template<class __InheritanceType, class __ReturnType = __InheritanceType>
	class NumeralOperators : public __InheritanceType	{
		//※以下の関数はこのクラスを使用する際に必須
		//template<class T> constexpr auto CAST(T&& v);
		//inline __ValueType GetValue() const noexcept;
		//inline void SetValue(const __ValueType& v) & noexcept;
		//inline void SetValue(const __ValueType&& v) & noexcept;

	private:
		using __MySelfType = NumeralOperators;

	protected:
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
		using __InheritanceType::__InheritanceType;

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}

		//単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		inline __ReturnType operator+() const { return __ReturnType(+this->GetValue()); }
		inline __ReturnType operator-() const { return __ReturnType(-this->GetValue()); }

		//算術演算子(Arithmetic)
		template<class T> inline __ReturnType operator+(T&& rhs) { return __ReturnType(this->GetValue() + this->CAST(rhs)); }
		template<class T> inline __ReturnType operator-(T&& rhs) { return __ReturnType(this->GetValue() - this->CAST(rhs)); }
		template<class T> inline __ReturnType operator*(T&& rhs) { return __ReturnType(this->GetValue() * this->CAST(rhs)); }
		template<class T> inline __ReturnType operator/(T&& rhs) { return __ReturnType(this->GetValue() / this->CAST(rhs)); }
		template<class T> inline __ReturnType operator%(T&& rhs) { return __ReturnType(mod(this->GetValue(), this->CAST(rhs))); }

		//複合代入演算子(Compound Assignment)
		template<class T> inline void operator+=(T&& rhs) { this->SetValue(this->GetValue() + this->CAST(rhs)); }
		template<class T> inline void operator-=(T&& rhs) { this->SetValue(this->GetValue() - this->CAST(rhs)); }
		template<class T> inline void operator*=(T&& rhs) { this->SetValue(this->GetValue() * this->CAST(rhs)); }
		template<class T> inline void operator/=(T&& rhs) { this->SetValue(this->GetValue() / this->CAST(rhs)); }
		template<class T> inline void operator%=(T&& rhs) { this->SetValue(mod(this->GetValue(), this->CAST(rhs))); }

		//後置インクリメント/デクリメント(Postfix Increment/Decrement)
		inline __ReturnType operator++(int) { auto z1 = this->GetValue(); this->SetValue(z1 + 1); return __ReturnType(z1); }
		inline __ReturnType operator--(int) { auto z1 = this->GetValue(); this->SetValue(z1 - 1); return __ReturnType(z1); }

		//前置インクリメント/デクリメント(Prefix Increment/Decremrnt)
		inline __MySelfType& operator++() { this->SetValue(this->GetValue() + 1); return *this; }
		inline __MySelfType& operator--() { this->SetValue(this->GetValue() - 1); return *this; }

		//論理否定演算子(Logical Not)
		inline bool operator!() const noexcept { return  this->GetValue() == 0; }

		//比較演算子(Compare)
		template<class T> inline bool operator==(T&& rhs) { return  this->GetValue() ==  this->CAST(rhs); }
		template<class T> inline bool operator!=(T&& rhs) { return  this->GetValue() !=  this->CAST(rhs); }
		template<class T> inline bool operator<=(T&& rhs) { return  this->GetValue() <=  this->CAST(rhs); }
		template<class T> inline bool operator< (T&& rhs) { return  this->GetValue() <   this->CAST(rhs); }
		template<class T> inline bool operator>=(T&& rhs) { return  this->GetValue() >=  this->CAST(rhs); }
		template<class T> inline bool operator> (T&& rhs) { return  this->GetValue() >   this->CAST(rhs); }

		//科学算術(Scientific Arithmetic)
		template<class T> inline __ReturnType pow(T&& rhs)	{ return __ReturnType(std::pow( this->GetValue(), this->CAST(rhs))); }
						  inline __ReturnType log()			{ return __ReturnType(std::log( this->GetValue())); }
		template<class T> inline __ReturnType log(T&& rhs)  { return __ReturnType(std::log( this->GetValue() / this->CAST(rhs))); }
						  inline __ReturnType abs()			{ return __ReturnType(std::abs( this->GetValue())); }

		//文字列変換(Text Conversion)
		inline std::string to_string(){ return std::to_string(this->GetValue()); }
	};

	/// <summary>
	/// 基本数字クラス
	/// </summary>
	/// <typeparam name="__EntityType"></typeparam>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __EntityType, class __ValueType>
	class BaseNumeral : public NumeralOperators<__EntityType, BaseNumeral<__EntityType, __ValueType>>
	{
	private:
		using __MySelfType = BaseNumeral;
		using __InheritanceType = NumeralOperators<__EntityType, BaseNumeral<__EntityType, __ValueType>>;

	public:
		using __InheritanceType::__InheritanceType;

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}
	};

	/// <summary>
	/// 数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	using Numeral = BaseNumeral<NumericEntity<__ValueType>, __ValueType>;
	namespace {
		using NumeralValueType = long double;
		static_assert(sizeof(Numeral<NumeralValueType>) == sizeof(NumeralValueType), "Numeral Size Error");
	}
#pragma endregion

#pragma region SIPrefixUnit

	/// <summary>
	/// SI接頭辞操作クラス
	/// </summary>
	/// <typeparam name="__InheritanceType"></typeparam>
	/// <typeparam name="__ReturnType"></typeparam>
	template<class __InheritanceType, class __ReturnType = __InheritanceType>
	using SIPrefixOperators = NumeralOperators<__InheritanceType, __ReturnType>;

	/// <summary>
	/// 基本SI接頭辞クラス
	/// 型を指定できる
	/// ※小数点のある型が望ましい
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseSIPrefix : public SIPrefixOperators<NumericEntity<__ValueType>, BaseSIPrefix<__ValueType>>
	{
	private:
		using __MySelfType = BaseSIPrefix;
		using __InheritanceType = SIPrefixOperators<NumericEntity<__ValueType>, BaseSIPrefix<__ValueType>>;

	protected:
		///// <summary>
		///// SI接頭辞単位実体クラス
		///// </summary>
		///// <typeparam name="__ValueType"></typeparam>
		///// <typeparam name="__Exp"></typeparam>
		template<int __Exp = 0>
		class SIPrefixUnitEntity
		{
		private:
			using __MySelfType = SIPrefixUnitEntity;
			BaseSIPrefix<__ValueType>* _pValue;

		protected:
			template<class T>
			constexpr auto CAST(T&& v) const { return static_cast<__ValueType>(v); }

			inline __ValueType GetValue() const noexcept
			{
				return static_cast<__ValueType>(*_pValue * std::pow(10, -__Exp));
			}
			inline void SetValue(const __ValueType& v) & noexcept
			{
				*_pValue = v * std::pow(10, __Exp);
			}
			inline void SetValue(const __ValueType&& v) & noexcept
			{
				SetValue(v);
			}

		public:
			//**********************************************************
			//暗黙的に宣言される
			SIPrefixUnitEntity() noexcept = delete;
			//SIPrefixUnitEntity(const __MySelfType&) noexcept = delete;
			//SIPrefixUnitEntity(__MySelfType&&) noexcept = delete;
			constexpr ~SIPrefixUnitEntity() noexcept = default;
			//**********************************************************
			constexpr SIPrefixUnitEntity(BaseSIPrefix<__ValueType>* other) noexcept : _pValue(other)
			{}

			//キャスト演算子(Cast)
			inline explicit operator __ValueType() const noexcept
			{
				return GetValue();
			}
		};
		template<int __Exp = 0>
		using BaseSIPrefixUnit = SIPrefixOperators<SIPrefixUnitEntity<__Exp>, BaseSIPrefix<__ValueType>>;

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseSIPrefix() noexcept = delete;
		//BaseSIPrefix(const __MySelfType&) noexcept = delete;
		//BaseSIPrefix(__MySelfType&&) noexcept = delete;
		~BaseSIPrefix() noexcept = default;
		//**********************************************************
		constexpr BaseSIPrefix(const __ValueType& init) noexcept : SIPrefixOperators<NumericEntity<__ValueType>, BaseSIPrefix<__ValueType>>(init)
		, Q(this)
		, R(this)
		, Y(this)
		, Z(this)
		, E(this)
		, P(this)
		, T(this)
		, G(this)
		, M(this)
		, k(this)
		, h(this)
		, da(this)
		, base(this)
		, d(this)
		, c(this)
		, m(this)
		, u(this)
		, n(this)
		, p(this)
		, f(this)
		, a(this)
		, z(this)
		, y(this)
		, r(this)
		, q(this)
		{}
		constexpr BaseSIPrefix(const __ValueType&& init = 0) noexcept : __MySelfType(init)
		{}
		constexpr BaseSIPrefix(const __MySelfType& other) noexcept : __MySelfType(other.GetValue())
		{}
		constexpr BaseSIPrefix(const __MySelfType&& other) noexcept : __MySelfType(other)
		{}

		BaseSIPrefixUnit< 30> Q;
		BaseSIPrefixUnit< 27> R;
		BaseSIPrefixUnit< 24> Y;
		BaseSIPrefixUnit< 21> Z;
		BaseSIPrefixUnit< 18> E;
		BaseSIPrefixUnit< 15> P;
		BaseSIPrefixUnit< 12> T;
		BaseSIPrefixUnit<  9> G;
		BaseSIPrefixUnit<  6> M;
		BaseSIPrefixUnit<  3> k;
		BaseSIPrefixUnit<  2> h;
		BaseSIPrefixUnit<  1> da;
		BaseSIPrefixUnit<  0> base;
		BaseSIPrefixUnit<- 1> d;
		BaseSIPrefixUnit<- 2> c;
		BaseSIPrefixUnit<- 3> m;
		BaseSIPrefixUnit<- 6> u;
		BaseSIPrefixUnit<- 9> n;
		BaseSIPrefixUnit<-12> p;
		BaseSIPrefixUnit<-15> f;
		BaseSIPrefixUnit<-18> a;
		BaseSIPrefixUnit<-21> z;
		BaseSIPrefixUnit<-24> y;
		BaseSIPrefixUnit<-27> r;
		BaseSIPrefixUnit<-30> q;
	
		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}
	};

	/// <summary>
	/// SI接頭辞クラス
	/// </summary>
	template<class __ValueType>
	using SIPrefix = BaseSIPrefix<__ValueType>;
	namespace {
		using SIPrefixType = long double;
		static_assert(sizeof(SIPrefix<SIPrefixType>) == sizeof(SIPrefixType) + sizeof(SIPrefix<SIPrefixType>*) * 25, "SIPrefix Size Error");
	}
#pragma endregion

#pragma region Specific
	/// <summary>
	/// 特殊数字実体クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class SpecificNumeralEntity
	{
	private:
		using __MySelfType = SpecificNumeralEntity;
		__ValueType _Value;

	protected:
		template<class T>
		constexpr auto CAST(T&& v) const { return static_cast<__ValueType>(v); }

		inline __ValueType GetValue() const noexcept
		{
			return _Value;
		}
		inline virtual void SetValue(const __ValueType& v) & noexcept
		{
			_Value = v;
		}
		inline virtual void SetValue(const __ValueType&& v) & noexcept
		{
			SetValue(v);
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		//SpecificNumeralEntity() noexcept = delete;
		//SpecificNumeralEntity(const __MySelfType&) noexcept = delete;
		//SpecificNumeralEntity(__MySelfType&&) noexcept = delete;
		constexpr ~SpecificNumeralEntity() noexcept = default;
		//**********************************************************
		constexpr SpecificNumeralEntity(const __ValueType & init) noexcept : _Value(init)
		{}
		constexpr SpecificNumeralEntity(const __ValueType && init = 0) noexcept : __MySelfType(init)
		{}
		template<class T> constexpr SpecificNumeralEntity(const T & other) noexcept : __MySelfType(CAST(other))
		{}
		template<class T> constexpr SpecificNumeralEntity(const T && other) noexcept : __MySelfType(other)
		{}

		//キャスト演算子(Cast)
		inline explicit operator __ValueType() const noexcept
		{
			return GetValue();
		}
	};

	/// <summary>
	/// 基本特殊数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	/// <typeparam name="__ReturnType"></typeparam>
	template<class __ValueType, class __ReturnType>
	class BaseSpecificNumeral : public NumeralOperators<SpecificNumeralEntity<__ValueType>, __ReturnType> 
	{
	private:
		using __MySelfType = BaseSpecificNumeral;
		using __InheritanceType = NumeralOperators<SpecificNumeralEntity<__ValueType>, __ReturnType>;

	public:
		using __InheritanceType::__InheritanceType;

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}
	};
#pragma endregion
}