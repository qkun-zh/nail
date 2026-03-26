<!--<script>-->
<!--    import {change_user_name} from "$lib/user/change_user_name.js";-->
<!--    import {asymmetric_encrypt} from "$lib/tool/encrypt.js";-->

<!--    import {get_login_status} from "$lib/user/get_login_status.js";-->
<!--    import {onMount} from "svelte";-->

<!--    let login_status = $state(false);-->

<!--    onMount(async () => {-->
<!--        login_status =get_login_status()-->
<!--    });-->

<!--    let new_user_name_and_jersey_number = $state("");-->
<!--    let email_address = $state("");-->
<!--    let verification_code = $state("");-->

<!--    let new_user_name_and_jersey_number_validity = $derived(judge_new_user_name_and_jersey_number_validity(new_user_name_and_jersey_number));-->
<!--    let email_address_validity = $derived(judge_email_address(email_address));-->
<!--    let verification_code_validity = $derived(judge_verification_code_validity(verification_code));-->

<!--    function judge_new_user_name_and_jersey_number_validity(user_name_and_jersey_number){-->
<!--        const regex = /^\w{1,16}#[0-9]{1,2}$/;-->
<!--        return regex.test(user_name_and_jersey_number);-->
<!--    }-->

<!--    function judge_email_address(email_address){-->
<!--        const regex = /^(?=.{6,254}$)[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?$/;-->
<!--        return regex.test(email_address);-->
<!--    }-->

<!--    function judge_verification_code_validity(verification_code){-->
<!--        const regex = /^[A-Za-z]{2}\d{4}$/;-->
<!--        return regex.test(verification_code);-->
<!--    }-->

<!--    let get_btn_countdown = $state(0);-->
<!--    let get_btn_countdown_timer = $state(null);-->
<!--    let change_user_name_and_jersey_number_btn_countdown = $state(0);-->
<!--    let change_user_name_and_jersey_number_btn_countdown_timer= $state(null);-->


<!--    function delay_get_btn() {-->
<!--        if (get_btn_countdown_timer) clearInterval(get_btn_countdown_timer);-->
<!--        get_btn_countdown = 64;-->
<!--        get_btn_countdown_timer = setInterval(() => {-->
<!--            get_btn_countdown&#45;&#45;;-->
<!--            if (get_btn_countdown <= 0) {-->
<!--                clearInterval(get_btn_countdown_timer);-->
<!--                get_btn_countdown_timer = null;-->
<!--                get_btn_countdown = 0;-->
<!--            }-->
<!--        }, 1000);-->
<!--    }-->

<!--    function delay_change_user_name_and_jersey_number_btn() {-->
<!--        if (change_user_name_and_jersey_number_btn_countdown_timer) clearInterval(change_user_name_and_jersey_number_btn_countdown_timer);-->
<!--        change_user_name_and_jersey_number_btn_countdown = 64;-->
<!--        change_user_name_and_jersey_number_btn_countdown_timer = setInterval(() => {-->
<!--            change_user_name_and_jersey_number_btn_countdown&#45;&#45;;-->
<!--            if (change_user_name_and_jersey_number_btn_countdown <= 0) {-->
<!--                clearInterval(change_user_name_and_jersey_number_btn_countdown_timer);-->
<!--                change_user_name_and_jersey_number_btn_countdown_timer = null;-->
<!--                change_user_name_and_jersey_number_btn_countdown = 0;-->
<!--            }-->
<!--        }, 1000);-->
<!--    }-->

<!--    let encrypted_email_address = $derived(asymmetric_encrypt(email_address))-->



<!--</script>-->


<!--{#if login_status===false}-->
<!--    <p>you have not logined. </p>-->
<!--{:else}-->
<!--    <div class="max-w-7xl mx-auto px-4 py-8 text-center">-->
<!--        <div class="flex flex-col items-center gap-4 border border-gray-300 rounded-lg p-4 md:p-8 w-full md:w-1/2 mx-auto">-->

<!--            <div class="relative w-full">-->
<!--                <input bind:value={new_user_name_and_jersey_number} maxlength=19 type="text" id="floating_username" class="block px-2.5 pb-2 pt-3.5 w-full text-xs md:text-sm text-gray-900 bg-transparent border-1 border-gray-300 appearance-none focus:outline-none focus:ring-0 focus:border-gray-700 peer rounded-lg" placeholder=" " />-->
<!--                <label for="floating_username" class="absolute text-xs md:text-sm text-gray-500 duration-300 transform -translate-y-3.5 md:-translate-y-4 scale-75 top-1.5 md:top-2 z-10 origin-[0] bg-white px-2 peer-focus:px-2 peer-focus:text-gray-700 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-1.5 md:peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-3.5 md:peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1 select-none cursor-default group">-->
<!--                    user_name#Jersey_number (new)-->
<!--                    <span class="hidden absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-1 bg-gray-200 text-gray-900 text-[11px] md:text-xs px-2 py-1 rounded whitespace-nowrap group-hover:block z-20">-->
<!--                    enter a string like user_name#Jersey_number<br>user_name is any combination of alphanumeric underscores, the length is limited to 16<br>jersey_number is a combination of any two single digits<br># must be between user_name and jersey_number-->
<!--                    <span class="absolute top-full left-1/2 -translate-x-1/2 translate-y-[-4px] w-2 h-2 rotate-45 bg-gray-200"></span>-->
<!--                </span>-->
<!--                </label>-->
<!--            </div>-->

<!--            <div class="relative w-full">-->
<!--                <input bind:value={email_address} type="text" maxlength=256 id="floating_message" class="block px-2.5 pb-2 pt-3.5 w-full text-xs md:text-sm text-gray-900 bg-transparent border-1 border-gray-300 appearance-none focus:outline-none focus:ring-0 focus:border-gray-700 peer rounded-lg" placeholder=" " />-->
<!--                <label for="floating_message" class="absolute text-xs md:text-sm text-gray-500 duration-300 transform -translate-y-3.5 md:-translate-y-4 scale-75 top-1.5 md:top-2 z-10 origin-[0] bg-white px-2 peer-focus:px-2 peer-focus:text-gray-700 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-1.5 md:peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-3.5 md:peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1 select-none cursor-default group">-->
<!--                    email_address-->
<!--                    <span class="hidden absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-1 bg-gray-200 text-gray-900 text-[11px] md:text-xs px-2 py-1 rounded whitespace-nowrap group-hover:block z-20">-->
<!--                    enter a string like local_part@domain_part<br>max length is 256-->
<!--                    <span class="absolute top-full left-1/2 -translate-x-1/2 translate-y-[-4px] w-2 h-2 rotate-45 bg-gray-200"></span>-->
<!--                </span>-->
<!--                </label>-->
<!--            </div>-->

<!--            <div class="w-full flex gap-2">-->
<!--                <div class="relative flex-1">-->
<!--                    <input bind:value={verification_code} maxlength=6 type="text" id="floating_verification" class="block px-2.5 pb-2 pt-3.5 w-full text-xs md:text-sm text-gray-900 bg-transparent border-1 border-gray-300 appearance-none focus:outline-none focus:ring-0 focus:border-gray-700 peer rounded-lg" placeholder=" " />-->
<!--                    <label for="floating_verification" class="absolute text-xs md:text-sm text-gray-500 duration-300 transform -translate-y-3.5 md:-translate-y-4 scale-75 top-1.5 md:top-2 z-10 origin-[0] bg-white px-2 peer-focus:px-2 peer-focus:text-gray-700 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-1.5 md:peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-3.5 md:peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1 select-none cursor-default group">-->
<!--                        verification_code-->
<!--                        <span class="hidden absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-1 bg-gray-200 text-gray-900 text-[11px] md:text-xs px-2 py-1 rounded whitespace-nowrap group-hover:block z-20">-->
<!--                        verification_code consists of two letters that are not case sensitive and four individual digits-->
<!--                        <span class="absolute top-full left-1/2 -translate-x-1/2 translate-y-[-4px] w-2 h-2 rotate-45 bg-gray-200"></span>-->
<!--                    </span>-->
<!--                    </label>-->
<!--                </div>-->
<!--                <button-->
<!--                        onclick={delay_get_btn}-->
<!--                        disabled={email_address_validity === false || get_btn_countdown > 0}-->
<!--                        class="whitespace-nowrap px-2.5 md:px-3 py-1.5 md:py-2 text-[11px] md:text-xs font-medium text-white bg-emerald-600 rounded-lg shadow-sm hover:bg-emerald-700 hover:shadow-md active:bg-emerald-800 active:shadow-sm active:scale-95 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2 select-none disabled:bg-gray-300 disabled:text-gray-500 disabled:cursor-not-allowed disabled:shadow-none disabled:hover:bg-gray-300 disabled:active:scale-100 disabled:focus:ring-gray-400"-->
<!--                >-->
<!--                    {get_btn_countdown > 0 ? `re-get will be allowed after ${get_btn_countdown}s` : 'get'}-->
<!--                </button>-->
<!--            </div>-->


<!--            <button-->
<!--                    onclick={()=>{change_user_name(); delay_change_user_name_and_jersey_number_btn();}}-->
<!--                    disabled={new_user_name_and_jersey_number_validity===false || email_address_validity===false || verification_code_validity===false || change_user_name_and_jersey_number_btn_countdown>0}-->
<!--                    class="w-full whitespace-nowrap px-6 md:px-8 py-3 md:py-3.5 text-sm md:text-base font-medium text-white bg-emerald-600 rounded-lg shadow-sm hover:bg-emerald-700 hover:shadow-md active:bg-emerald-800 active:shadow-sm active:scale-95 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2 select-none disabled:bg-gray-300 disabled:text-gray-500 disabled:cursor-not-allowed disabled:shadow-none disabled:hover:bg-gray-300 disabled:active:scale-100 disabled:focus:ring-gray-400"-->
<!--            >-->
<!--                {change_user_name_and_jersey_number_btn_countdown > 0 ? `re-change will be allowed after ${change_user_name_and_jersey_number_btn_countdown}s` : 'change_user_name'}-->
<!--            </button>-->
<!--        </div>-->
<!--    </div>-->
<!--{/if}-->


