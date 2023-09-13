#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod ceres_governance {

    use ink::storage::Mapping;
    use scale::{Decode, Encode};
    use ink::prelude::string::String;

    #[derive(Encode, Decode, Default, PartialEq, Eq)]
    #[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct VotingInfo {
        /// Voting option
        voting_option: u32,
        /// Number of votes
        number_of_votes: Balance,
        /// Ceres withdrawn
        ceres_withdrawn: bool,
    }

    #[derive(Encode, Decode, Default, PartialEq, Eq, Debug)]
    #[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct PollInfo {
        /// Number of options
        pub number_of_options: u32,
        /// Poll start timestamp
        pub poll_start_timestamp: Timestamp,
        /// Poll end timestamp
        pub poll_end_timestamp: Timestamp,
    }

    #[ink(storage)]
    pub struct CeresGovernance {
        poll_data: Mapping<String, PollInfo>,
        voting: Mapping<(String, AccountId), VotingInfo>,  
    }

    // Events
    #[ink(event)]
    pub struct Voted {
        #[ink(topic)]
        poll_id: String,
        #[ink(topic)]
        voter: AccountId,
        voting_option: u32,
        number_of_votes: Balance,
    } 

    #[ink(event)]
    pub struct PollCreated {
        #[ink(topic)]
        poll_id: String,
        #[ink(topic)]
        number_of_options: u32,
        poll_start_timestamp: Timestamp,
        poll_end_timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct FundsWithdrawn {
        #[ink(topic)]
        voter: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Invalid votes
        InvalidVotes,
        /// Poll is finished
        PollIsFinished,
        /// Poll is not started
        PollIsNotStarted,
        /// Not enough funds
        NotEnoughFunds,
        /// Invalid number of option
        InvalidNumberOfOption,
        /// Vote denied
        VoteDenied,
        /// Invalid start timestamp
        InvalidStartTimestamp,
        /// Invalid end timestamp
        InvalidEndTimestamp,
        /// Poll is not finished
        PollIsNotFinished,
        /// Invalid number of votes
        InvalidNumberOfVotes,
        /// Funds already withdrawn,
        FundsAlreadyWithdrawn,
        /// Poll id already exists
        PollIdAlreadyExists,
        /// Poll does not exist
        PollDoesNotExist,
    }
    
    impl CeresGovernance {

        #[ink(constructor)]
        // Creat a new instance of the contract passing the address of the Ceres token
        pub fn new() -> Self {
            Self {
                poll_data: Mapping::new(),
                voting: Mapping::new(),  
            }
        }

        #[ink(message)]
        pub fn create_poll(
            &mut self,
            poll_id: String,
            number_of_options: u32,
            poll_start_timestamp: Timestamp,
            poll_end_timestamp: Timestamp,
        ) -> Result<(), Error> {

            let current_timestamp = self.env().block_timestamp();
            let poll_info = self.poll_data.get(&poll_id).unwrap_or_default();

            if poll_info.number_of_options != 0 {
                return Err(Error::PollIdAlreadyExists);
            }
            
            if number_of_options < 2 {
                return Err(Error::InvalidNumberOfOption)
            }

            if poll_start_timestamp < current_timestamp {
                return Err(Error::InvalidStartTimestamp)
            }

            if poll_end_timestamp <= poll_start_timestamp {
                return Err(Error::InvalidEndTimestamp)
            }

            let poll_info = PollInfo {
                number_of_options,
                poll_start_timestamp,
                poll_end_timestamp,
            };

            self.poll_data.insert(&poll_id, &poll_info);

            self.env().emit_event(PollCreated {
                poll_id: poll_id.clone(),
                number_of_options,
                poll_start_timestamp,
                poll_end_timestamp,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            poll_id: String,
            voting_option: u32,
            number_of_votes: Balance,
        ) -> Result<(), Error>{
            let caller = self.env().caller();

            if number_of_votes <= 0 {
                return Err(Error::InvalidNumberOfVotes)
            }

            let poll_info = self.poll_data.get(&poll_id).unwrap_or_default();
            let current_timestamp = self.env().block_timestamp();       

            if current_timestamp < poll_info.poll_start_timestamp {
                return Err(Error::PollIsNotStarted)
            }

            if current_timestamp > poll_info.poll_end_timestamp {
                return Err(Error::PollIsFinished);
            }
    
            if voting_option > poll_info.number_of_options{
                return Err(Error::InvalidNumberOfOption)
            }

            let mut voting_info = self.voting.get(&(poll_id.clone(), caller)).unwrap_or_default();

            if voting_info.voting_option == 0 {
                voting_info.voting_option = voting_option;                
            } else {
                if voting_info.voting_option != voting_option {
                    return Err(Error::VoteDenied)
                }
            }

            voting_info.number_of_votes += number_of_votes;    
                
            self.voting.insert(&(poll_id.clone(), caller), &voting_info); 

            self.env().emit_event(Voted {
                poll_id: poll_id.clone(),
                voter: caller,
                voting_option,
                number_of_votes,
            });           

            Ok(().into())
        }

        #[ink(message)]
        pub fn withdrawn(
            &mut self,
            poll_id: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let poll_info = self.poll_data.get(&poll_id).unwrap_or_default();
            let current_timestamp = self.env().block_timestamp();

            if poll_info.number_of_options == 0 {
                return Err(Error::PollDoesNotExist)
            }

            if current_timestamp < poll_info.poll_end_timestamp {
                return Err(Error::PollIsNotFinished)
            }

            let mut voting_info = self.voting.get(&(poll_id.clone(), caller)).unwrap_or_default();

            if voting_info.number_of_votes == 0 {
                return Err(Error::InvalidVotes)
            }

            if voting_info.ceres_withdrawn == true {
                return Err(Error::FundsAlreadyWithdrawn)
            }

            voting_info.ceres_withdrawn = true;
            self.voting.insert(&(poll_id.clone(), caller), &voting_info);

            self.env().emit_event(FundsWithdrawn {
                voter: caller,
                amount: voting_info.number_of_votes,
            });

            Ok(().into())
        }

        #[ink(message)]
        pub fn get_poll_info(
            &self,
            poll_id: String,
        ) -> Result<PollInfo, Error> {
            let poll_info = self.poll_data.get(&poll_id).unwrap_or_default();

            if poll_info.number_of_options == 0 {
                return Err(Error::PollDoesNotExist)
            }

            Ok(poll_info)
        } 
        
    }
}
