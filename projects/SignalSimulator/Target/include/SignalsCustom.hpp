#pragma once

#include <cassert>
#include <vector>

namespace Signals {
    template<class __SignalType>
    class SignalCustom
    {
	private:
		using __MySelfType = SignalCustom;

		inline static std::vector<__SignalType*> signalList = {};

		static void DebuggingTimestamp(__SignalType& pObj) noexcept
		{
			GeneralPurposeTimer::DateFormat::UTC date;
			std::cout << "【Name:" << pObj.name() << "-Group:" << pObj.group() << "-Comment:" << pObj.comment() << "】";
			std::cout << date.format() << ":";
		}
		static void DebuggingTimestampForRegistered(__SignalType& pObj) noexcept
		{
			DebuggingTimestamp(pObj);
			std::cout << "Registered" << std::endl;
		}
		static void DebuggingTimestampForDelete(__SignalType& pObj) noexcept
		{
			DebuggingTimestamp(pObj);
			std::cout << "Delete" << std::endl;
		}

    public:
		//**********************************************************
		//Signal宣言される
		SignalCustom() noexcept = delete;
		SignalCustom(const __MySelfType&) noexcept = delete;
		SignalCustom(__MySelfType&&) noexcept = delete;
		constexpr ~SignalCustom() noexcept = default;
		//**********************************************************

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************

        static void registeredSignalObjects(__SignalType* pObj) noexcept
        {
			//DebuggingTimestampForRegistered(*pObj);

			signalList.push_back(pObj);
        }
		static void deleteSignalObject(__SignalType* pObj) noexcept
		{
			//DebuggingTimestampForDelete(*pObj);

			for (auto it = signalList.begin(); it != signalList.end();) {
				// 条件一致した要素を削除する
				if (*it == pObj) {
					// 削除された要素の次を指すイテレータが返される。
					it = signalList.erase(it);
					return;
				}
				// 要素削除をしない場合に、イテレータを進める
				++it;
			}
			assert(false);
		}
    };
}