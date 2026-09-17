
#pragma once

#include "UnitOfNumber.hpp"

namespace UnitOfNumber {
	//科学定数
	static const struct ScientificPostulates {
		using Type = long double;
		using SIType = SIPrefix<Type>;

		ScientificPostulates()
		: C  (2.99792458      * std::pow(10, + 8)), Co(C)
		, g  (9.80665                            )
		, G  (6.6742          * std::pow(10, -11))
		, me (9.10938356      * std::pow(10, -31))
		, mp (1.672621777     * std::pow(10, -27))
		, mn (1.67492728      * std::pow(10, -27))
		, ou (1.660538921     * std::pow(10, -27))
		, e0 (8.854187817     * std::pow(10, -12))
		, u0 (1.256637061     * std::pow(10, - 6))
		, h  (6.62606957      * std::pow(10, -34))
		, e  (1.602176565     * std::pow(10, -19))
		, Rif(1.0973731568539 * std::pow(10, + 7))
		, NA (6.02214129      * std::pow(10, +23)), L(NA)
		, Vm (2.2413996       * std::pow(10, - 2))
		, F  (9.64853365      * std::pow(10, + 4))
		, R  (8.3144621                          )
		, k  (1.3806488       * std::pow(10, -23))
		, t  (273.15                             )
		, KJ (483597.870      * std::pow(10, + 9))
		, atm(1.02325         * std::pow(10, + 5))
		{}

		/// <summary>
		/// 空中の光の速さ
		/// =2.99792458×10^8[m・s^-1]
		/// </summary>
		SIType C, Co;

		/// <summary>
		/// 重力加速度
		/// =9.80665[m・s^-2]
		/// </summary>
		SIType g;

		/// <summary>
		/// 万有引力定数
		/// =6.6742×10^-11[m3・kg^-1・s^-2]
		/// </summary>
		SIType G;

		/// <summary>
		/// 電子の質量
		/// =9.10938356×10^-31[kg]
		/// </summary>
		SIType me;

		/// <summary>
		/// 陽子の質量
		/// =1.672621777×10^-27[kg]
		/// </summary>
		SIType mp;

		/// <summary>
		/// 中性子の質量
		/// =1.67492728×10^-27[kg]
		/// </summary>
		SIType mn;

		/// <summary>
		/// 原子質量単位
		/// =1.660538921×10^-27[kg]
		/// </summary>
		SIType ou;

		/// <summary>
		/// 真空の誘電率
		/// =8.854187817×10^-12[F・m^-1]
		/// </summary>
		SIType e0;

		/// <summary>
		/// 真空の透磁率
		/// =1.256637061×10^-6[N・A^-2]
		/// </summary>
		SIType u0;

		/// <summary>
		/// プランク定数
		/// =6.62606957×10^-34[J・s]
		/// </summary>
		SIType h;

		/// <summary>
		/// 電気素量
		/// =1.602176565×10^-19[C]
		/// </summary>
		SIType e;

		/// <summary>			
		/// リュードベリ定数	
		/// =1.0973731568539×10^7[m^-1]
		/// </summary>
		SIType Rif;

		/// <summary>	
		/// アボガドロ数
		/// =6.02214129×10^23[mol^-1]
		/// </summary>
		SIType NA, L;

		/// <summary>	
		/// 理想気体の標準体積	
		/// =2.2413996×10^-2[m3・mol^-1]
		/// </summary>
		SIType Vm;

		/// <summary>	
		/// ファラデー定数		
		/// =9.64853365×10^4[C・mol^-1]
		/// </summary>	
		SIType F;

		/// <summary>	
		/// １モルの気体定数	
		/// =8.3144621[J・mol^-1・K^-1]
		/// </summary>		
		SIType R;

		/// <summary>	
		/// ボルツマン定数		
		/// =1.3806488×10^-23[J・K^-1]
		/// </summary>		
		SIType k;

		/// <summary>	
		/// セルシウス温度		
		/// =273.15[K]
		/// </summary>
		SIType t;

		/// <summary>	
		/// ジョセフソン定数	
		/// =483597.870×10^9[Hz・V^-1]
		/// </summary>		
		SIType KJ;

		/// <summary>	
		/// 標準大気圧			
		/// =1.02325×10^5[Pa]
		/// </summary>				
		SIType atm;
	}SP;
}
